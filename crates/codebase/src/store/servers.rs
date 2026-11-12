//! One job: the person's language-server settings persistence.

use std::collections::HashSet;

use super::CodeStore;
use core_db::StorageError;

/// One server the person added, as `code_server_added` holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AddedRow {
    pub(crate) id: String,
    pub(crate) command: String,
    pub(crate) args: Vec<String>,
    pub(crate) extensions: Vec<String>,
}

impl CodeStore {
    /// The ids of the servers the person switched off.
    pub(in super::super) async fn servers_off(&self) -> Result<HashSet<String>, StorageError> {
        let rows: Vec<String> =
            sqlx::query_scalar("SELECT server_id FROM code_server_off ORDER BY server_id")
                .fetch_all(self.storage.reader())
                .await?;
        Ok(rows.into_iter().collect())
    }

    /// The servers the person added, by id.
    pub(in super::super) async fn servers_added(&self) -> Result<Vec<AddedRow>, StorageError> {
        let rows: Vec<(String, String, String, String)> = sqlx::query_as(
            "SELECT id, command, args, extensions FROM code_server_added ORDER BY id",
        )
        .fetch_all(self.storage.reader())
        .await?;
        Ok(rows
            .into_iter()
            .map(|(id, command, args, extensions)| AddedRow {
                id,
                command,
                args: serde_json::from_str(&args).unwrap_or_default(),
                extensions: serde_json::from_str(&extensions).unwrap_or_default(),
            })
            .collect())
    }

    /// The extensions of the files the project's last listing found, as written.
    pub(in super::super) async fn file_extensions(
        &self,
        project: &core_projects::ProjectId,
    ) -> Result<HashSet<String>, StorageError> {
        let paths: Vec<String> = sqlx::query_scalar(
            "SELECT path FROM code_inventory WHERE project_id = ? AND kind = 'file'
              ORDER BY path_key",
        )
        .bind(project.as_str())
        .fetch_all(self.storage.reader())
        .await?;
        Ok(paths
            .iter()
            .filter_map(|p| {
                let name = p.rsplit('/').next()?;
                let dot = name.rfind('.').filter(|&i| i > 0)?;
                Some(name[dot..].to_string())
            })
            .collect())
    }

    /// Test only: stores an added server with no command record.
    #[cfg(feature = "test-support")]
    pub(in super::super) async fn add_server_row(&self, row: AddedRow) -> Result<(), StorageError> {
        let at = core_db::Storage::now();
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    sqlx::query(
                        "INSERT INTO code_server_added (id, command, args, extensions, at)
                         VALUES (?,?,?,?,?)",
                    )
                    .bind(&row.id)
                    .bind(&row.command)
                    .bind(serde_json::to_string(&row.args).unwrap_or_default())
                    .bind(serde_json::to_string(&row.extensions).unwrap_or_default())
                    .bind(&at)
                    .execute(&mut *conn)
                    .await?;
                    Ok(())
                })
            })
            .await
    }
}
