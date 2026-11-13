//! One job: the person's language-server settings persistence.

use std::collections::HashSet;

use super::CodeStore;
use core_db::{Actor, CommandContext, JournalEntry, Storage, StorageError};

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
}

/// One change to the person's server settings, as one command writes it.
#[derive(Debug, Clone)]
pub(crate) enum ServerChange {
    /// A server switched off (`true`) or back on.
    Off(String, bool),
    /// A server added; refused when its id is already added.
    Add(AddedRow),
    /// An added server removed; refused when no such one was added.
    Remove(String),
}

impl CodeStore {
    /// One write: classify (a replay answers at once), the change, its journal event
    /// and the command record. `Ok(Err(..))` is a refusal with nothing written.
    pub(in super::super) async fn change_servers(
        &self,
        ctx: &CommandContext,
        change: ServerChange,
    ) -> Result<Result<(), String>, StorageError> {
        let (ctx, ts) = (ctx.clone(), Storage::now());
        self.storage
            .write_txn(move |conn| {
                Box::pin(async move {
                    if CommandContext::classify(conn, &ctx, "Code", "servers")
                        .await?
                        .is_some()
                    {
                        return Ok(Ok(()));
                    }
                    let (event, payload, outcome) = match &change {
                        ServerChange::Off(id, off) => {
                            if *off {
                                sqlx::query(
                                    "INSERT INTO code_server_off (server_id, at) VALUES (?, ?)
                                     ON CONFLICT (server_id) DO NOTHING",
                                )
                                .bind(id)
                                .bind(&ts)
                                .execute(&mut *conn)
                                .await?;
                            } else {
                                sqlx::query("DELETE FROM code_server_off WHERE server_id = ?")
                                    .bind(id)
                                    .execute(&mut *conn)
                                    .await?;
                            }
                            let payload = serde_json::json!({ "server": id, "off": off });
                            ("CodeServerSwitched", payload, id.clone())
                        }
                        ServerChange::Add(row) => {
                            let known: Option<String> =
                                sqlx::query_scalar("SELECT id FROM code_server_added WHERE id = ?")
                                    .bind(&row.id)
                                    .fetch_optional(&mut *conn)
                                    .await?;
                            if known.is_some() {
                                return Ok(Err(format!("the server {} is already added", row.id)));
                            }
                            sqlx::query(
                                "INSERT INTO code_server_added (id, command, args, extensions, at)
                                 VALUES (?,?,?,?,?)",
                            )
                            .bind(&row.id)
                            .bind(&row.command)
                            .bind(serde_json::to_string(&row.args).unwrap_or_default())
                            .bind(serde_json::to_string(&row.extensions).unwrap_or_default())
                            .bind(&ts)
                            .execute(&mut *conn)
                            .await?;
                            let payload = serde_json::json!({
                                "server": row.id, "command": row.command,
                                "args": row.args, "extensions": row.extensions,
                            });
                            ("CodeServerAdded", payload, row.id.clone())
                        }
                        ServerChange::Remove(id) => {
                            let gone = sqlx::query("DELETE FROM code_server_added WHERE id = ?")
                                .bind(id)
                                .execute(&mut *conn)
                                .await?
                                .rows_affected();
                            if gone == 0 {
                                return Ok(Err(format!("no server {id} was added")));
                            }
                            sqlx::query("DELETE FROM code_server_off WHERE server_id = ?")
                                .bind(id)
                                .execute(&mut *conn)
                                .await?;
                            let payload = serde_json::json!({ "server": id });
                            ("CodeServerRemoved", payload, id.clone())
                        }
                    };
                    let entry = JournalEntry::new(event, Actor::user(&ctx.principal_id))
                        .with_payload(payload);
                    JournalEntry::append_event(conn, entry, &ts).await?;
                    CommandContext::record_command(
                        conn,
                        &ctx,
                        "Code",
                        "servers",
                        "CodeServer",
                        &outcome,
                        &ts,
                    )
                    .await?;
                    Ok(Ok(()))
                })
            })
            .await
    }
}
