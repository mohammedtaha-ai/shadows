//! One job: the stored verdicts, private to the code record.

use std::sync::Arc;

use core_db::{Storage, StorageError, Tx};
use core_projects::ProjectId;

use crate::{CodeRecord, CodeRecordError, Verdict};

#[derive(Clone)]
pub(crate) struct CodeRecordStore {
    storage: Arc<Storage>,
}

impl CodeRecordStore {
    pub(super) fn new(storage: Arc<Storage>) -> Self {
        Self { storage }
    }

    /// Stores the verdict under `key`, replacing one stored before.
    pub(super) async fn put(
        &self,
        key: &str,
        project: &ProjectId,
        verdict: &Verdict,
    ) -> Result<(), CodeRecordError> {
        let (key, project, json, ts) = (
            key.to_string(),
            project.clone(),
            serde_json::to_string(verdict)?,
            Storage::now(),
        );
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    sqlx::query(
                        "INSERT INTO code_record_verdict (attempt_id, project_id, verdict_json, verified_at)
                         VALUES (?, ?, ?, ?)
                         ON CONFLICT (attempt_id) DO UPDATE SET
                           project_id = excluded.project_id,
                           verdict_json = excluded.verdict_json,
                           verified_at = excluded.verified_at",
                    )
                    .bind(&key)
                    .bind(project.as_str())
                    .bind(&json)
                    .bind(&ts)
                    .execute(&mut *conn)
                    .await?;
                    Ok(())
                })
            })
            .await?;
        Ok(())
    }

    pub(super) async fn verdict(&self, key: &str) -> Result<Option<Verdict>, CodeRecordError> {
        let json: Option<String> =
            sqlx::query_scalar("SELECT verdict_json FROM code_record_verdict WHERE attempt_id = ?")
                .bind(key)
                .fetch_optional(self.storage.reader())
                .await?;
        Ok(json.map(|j| serde_json::from_str(&j)).transpose()?)
    }
}

impl CodeRecord {
    /// Every verdict of a removed project, inside the removal's own write.
    pub async fn delete_project_in(conn: &mut Tx, project: &ProjectId) -> Result<(), StorageError> {
        sqlx::query("DELETE FROM code_record_verdict WHERE project_id = ?")
            .bind(project.as_str())
            .execute(&mut *conn)
            .await?;
        Ok(())
    }
}
